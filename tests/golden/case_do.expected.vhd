library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_case_do_example is
  port (
    gl_p0_clk : in std_logic;
    gl_p1_selector : in unsigned(1 downto 0);
    gl_p2_input : in unsigned(7 downto 0);
    gl_p3_value : out unsigned(7 downto 0);
    gl_p4_busy : out std_logic
  );
end entity gl_m0_case_do_example;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_case_do_example is
  signal gl_s3_value : unsigned(7 downto 0);
  signal gl_s4_busy : std_logic;
  signal gl_s5_stored : unsigned(7 downto 0) := resize(unsigned'(x"0000000000000000"), 8);
  signal gl_s6_active : std_logic := '0';
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
    gl_s3_value <= gl_s5_stored;
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s4_busy <= gl_s6_active;
  end process gl_comb_1;
  gl_seq_0 : process(gl_p0_clk)
    variable gl_tmp_0 : unsigned(1 downto 0);
  begin
    if rising_edge(gl_p0_clk) then
      gl_tmp_0 := gl_p1_selector;
      if (gl_tmp_0 = resize(unsigned'(x"0000000000000000"), 2)) then
        gl_s5_stored <= gl_p2_input;
        gl_s6_active <= '0';
      else
        if (gl_tmp_0 = resize(unsigned'(x"0000000000000001"), 2)) then
          gl_s5_stored <= shift_left(gl_p2_input, 1);
          gl_s6_active <= '1';
        else
          gl_s5_stored <= resize(unsigned'(x"0000000000000000"), 8);
          gl_s6_active <= '0';
        end if;
      end if;
    end if;
  end process gl_seq_0;
  gl_p3_value <= gl_s3_value;
  gl_p4_busy <= gl_s4_busy;
end architecture rtl;
