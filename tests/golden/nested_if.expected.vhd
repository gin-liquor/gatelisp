library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_nested_if is
  port (
    gl_p0_select : in std_logic;
    gl_p1_a : in unsigned(7 downto 0);
    gl_p2_b : in unsigned(7 downto 0);
    gl_p3_c : in unsigned(7 downto 0);
    gl_p4_y : out unsigned(7 downto 0)
  );
end entity gl_m0_nested_if;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_nested_if is
  signal gl_s4_y : unsigned(7 downto 0);
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
    variable gl_tmp_0 : unsigned(7 downto 0);
  begin
    if (gl_p0_select = '1') then
      gl_tmp_0 := gl_p2_b;
    else
      gl_tmp_0 := gl_p3_c;
    end if;
    gl_s4_y <= (gl_p1_a + gl_tmp_0);
  end process gl_comb_0;
  gl_p4_y <= gl_s4_y;
end architecture rtl;
