library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_clock_edges is
  port (
    gl_p0_clk : in std_logic;
    gl_p1_reset : in std_logic;
    gl_p2_input : in std_logic;
    gl_p3_rising_value : out std_logic;
    gl_p4_falling_value : out std_logic
  );
end entity gl_m0_clock_edges;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_clock_edges is
  signal gl_s3_rising_value : std_logic;
  signal gl_s4_falling_value : std_logic;
  signal gl_s5_rising_reg : std_logic := '0';
  signal gl_s6_falling_reg : std_logic := '0';
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s3_rising_value <= gl_s5_rising_reg;
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s4_falling_value <= gl_s6_falling_reg;
  end process gl_comb_1;
  gl_seq_0 : process(gl_p0_clk)
  begin
    if rising_edge(gl_p0_clk) then
      if (gl_p1_reset = '1') then
        gl_s5_rising_reg <= '0';
      else
        gl_s5_rising_reg <= gl_p2_input;
      end if;
    end if;
  end process gl_seq_0;
  gl_seq_1 : process(gl_p0_clk, gl_p1_reset)
  begin
    if (gl_p1_reset = '1') then
      gl_s6_falling_reg <= '0';
    else
      if falling_edge(gl_p0_clk) then
        gl_s6_falling_reg <= gl_p2_input;
      end if;
    end if;
  end process gl_seq_1;
  gl_p3_rising_value <= gl_s3_rising_value;
  gl_p4_falling_value <= gl_s4_falling_value;
end architecture rtl;
