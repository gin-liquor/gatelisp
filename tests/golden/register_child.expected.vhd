library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_counter_child is
  port (
    gl_p0_clk : in std_logic;
    gl_p1_rst : in std_logic;
    gl_p2_q : out unsigned(7 downto 0)
  );
end entity gl_m0_counter_child;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_counter_child is
  signal gl_s2_q : unsigned(7 downto 0);
  signal gl_s3_count : unsigned(7 downto 0) := resize(unsigned'(x"0000000000000000"), 8);
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
    gl_s2_q <= gl_s3_count;
  end process gl_comb_0;
  gl_seq_0 : process(gl_p0_clk)
  begin
    if rising_edge(gl_p0_clk) then
      if (gl_p1_rst = '1') then
        gl_s3_count <= resize(unsigned'(x"0000000000000000"), 8);
      else
        gl_s3_count <= (gl_s3_count + resize(unsigned'(x"0000000000000001"), 8));
      end if;
    end if;
  end process gl_seq_0;
  gl_p2_q <= gl_s2_q;
end architecture rtl;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m1_counter_top is
  port (
    gl_p4_clk : in std_logic;
    gl_p5_rst : in std_logic;
    gl_p6_q : out unsigned(7 downto 0)
  );
end entity gl_m1_counter_top;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m1_counter_top is
  signal gl_s6_q : unsigned(7 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_i0_counter0 : entity work.gl_m0_counter_child
    port map (
      gl_p0_clk => gl_p4_clk,
      gl_p1_rst => gl_p5_rst,
      gl_p2_q => gl_s6_q
    );
  gl_p6_q <= gl_s6_q;
end architecture rtl;
